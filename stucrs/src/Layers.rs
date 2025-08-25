use crate::core_new::{ArrayDToRcVariable, F32ToRcVariable};
use crate::core_new::{RcVariable, Variable};
use crate::functions_new as F;
use ndarray::{array, Array, ArrayBase, Dim, IxDyn, OwnedRepr};
use ndarray_rand::rand_distr::{Normal, StandardNormal, Uniform};
use ndarray_rand::RandomExt;
use std::array;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::process::Output;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU32, Ordering};

/// Variableや関数たちにidを付けるための値
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

pub trait Layer: Debug {
    fn set_params(&mut self, param: &RcVariable);

    fn get_input(&self) -> RcVariable;
    fn get_output(&self) -> RcVariable;
    fn get_generation(&self) -> i32;
    fn get_id(&self) -> u32;
    fn params(&self);
    fn cleargrad(&mut self);
}

#[derive(Debug, Clone)]
pub struct Linear {
    input: Option<Weak<RefCell<Variable>>>,
    output: Option<Weak<RefCell<Variable>>>,
    out_size: u32,
    w_id: Option<u32>,
    b_id: Option<u32>,
    params: HashMap<u32, RcVariable>,
    generation: i32,
    id: u32,
}

impl Layer for Linear {
    fn set_params(&mut self, param: &RcVariable) {
        self.params.insert(param.id(), param.clone());
    }
    fn get_input(&self) -> RcVariable {
        let input = self
            .input
            .as_ref()
            .unwrap()
            .upgrade()
            .as_ref()
            .unwrap()
            .clone();
        RcVariable(input)
    }

    fn get_output(&self) -> RcVariable {
        let output;
        output = self
            .output
            .as_ref()
            .unwrap()
            .upgrade()
            .as_ref()
            .unwrap()
            .clone();

        RcVariable(output)
    }

    fn get_generation(&self) -> i32 {
        self.generation
    }
    fn get_id(&self) -> u32 {
        self.id
    }
    fn params(&self) {
        for (_id, param) in self.params.iter() {
            println!("param = {:?}", param);
        }
    }

    fn cleargrad(&mut self) {
        for (_id, param) in self.params.iter_mut() {
            param.cleargrad();
        }
    }
}

impl Linear {
    pub fn call(&mut self, input: &RcVariable) -> RcVariable {
        // inputのvariableからdataを取り出す

        let output = self.forward(input);

        //ここから下の処理はbackwardするときだけ必要。

        //　inputsを覚える
        self.input = Some(input.downgrade());

        self.generation = input.generation();

        //  outputを弱参照(downgrade)で覚える
        self.output = Some(output.downgrade());

        output
    }

    fn forward(&mut self, x: &RcVariable) -> RcVariable {
        if let None = &self.w_id {
            let i = x.data().shape()[1];
            let o = self.out_size as usize;
            let i_f32 = i as f32;

            let w_data: ArrayBase<OwnedRepr<f32>, Dim<[usize; 2]>> =
                &Array::random((i, o), StandardNormal) * ((1.0f32 / i_f32).sqrt());

            let w = w_data.rv();

            self.w_id = Some(w.id());
            self.set_params(&w.clone());
        }

        let w_id = self.w_id.unwrap();
        let w = self.params.get(&w_id).unwrap();

        let b;
        if let Some(b_id_data) = self.b_id {
            b = self.params.get(&b_id_data).cloned();
        } else {
            b = None;
        }

        let y = F::linear_simple(&x, &w, &b);

        y
    }

    pub fn new(out_size: u32, biased: bool, opt_in_size: Option<u32>) -> Rc<RefCell<Self>> {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let linear = Rc::new(RefCell::new(Self {
            input: None,
            output: None,
            out_size: out_size,
            w_id: None,
            b_id: None,
            params: HashMap::new(),
            generation: 0,
            id: id,
        }));

        //in_sizeが設定されていたら、ここでWを作成
        //されていない場合は後で作成
        if let Some(in_size) = opt_in_size {
            let i = in_size as usize;
            let o = out_size as usize;

            let i_f32 = in_size as f32;

            let w_data: ArrayBase<OwnedRepr<f32>, Dim<[usize; 2]>> =
                &Array::random((i, o), StandardNormal) * ((1.0f32 / i_f32).sqrt());

            let w = w_data.rv();

            linear.borrow_mut().w_id = Some(w.id());
            linear.borrow_mut().set_params(&w.clone());
        }

        if biased == true {
            let b = Array::zeros(out_size as usize).rv();
            linear.borrow_mut().b_id = Some(b.id());
            linear.borrow_mut().set_params(&b.clone());
        }

        linear
    }

    pub fn update_params(&mut self, lr: f32) {
        for (_id, param) in self.params.iter() {
            let param_data = param.data();
            let current_grad = param.grad().as_ref().unwrap().data();
            param.0.borrow_mut().data = param_data - lr * current_grad;
        }
    }
}
