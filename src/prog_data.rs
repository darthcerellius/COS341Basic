use std::collections::{HashMap, LinkedList};

pub struct ProgramData {
    code: Vec<String>,
    vars: HashMap<String, String>,
    stack: LinkedList<String>,
    index: usize,
}

impl ProgramData {

    pub fn new(code: Vec<String>,
               vars: HashMap<String, String>,
               stack: LinkedList<String>,
               index: usize) -> Self {
        ProgramData{code, vars, stack, index}
    }

    pub fn get_code(&self) -> Option<String> {
        self.code.get(self.index).cloned()
    }

    pub fn set_index(&mut self, new_index: usize) {
        self.index = new_index;
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn next_line(&mut self) {
        self.index += 1;
    }

    pub fn push(&mut self, data: String) {
        self.stack.push_front(data);
    }

    pub fn pop(&mut self) -> Option<String> {
        self.stack.pop_front()
    }

    pub fn get_var(&self, key: &String) -> Option<&String> {
        self.vars.get(&*key)
    }

    pub fn set_var(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    pub fn set_var_to_var(&mut self, lhs_key: String, rhs_key: String) {
        self.vars.insert(lhs_key, (*self.get_var(&rhs_key).unwrap()).parse().unwrap());
    }

    pub fn contains_var(&self, key: &String) -> bool {
        self.vars.contains_key(&*key)
    }

    pub fn code_size(&self) -> usize {
        self.code.len()
    }
}