#[derive(Default)]
pub struct Dispatcher<T: Event> {
    subbed_fns: Vec<fn(&T)>,
    subbed_methods: Vec<(usize, usize)>, // slf_addr, fn_addr
}

impl<T: Event> Dispatcher<T> {
    pub fn new() -> Self {
        Self {
            subbed_fns: Vec::new(),
            subbed_methods: Vec::new(),
        }
    }

    pub fn post(&mut self, e: &T) {
        for sub in self.subbed_fns.iter() {
            sub(e);
        }
        for &(slf, sub) in self.subbed_methods.iter() {
            let sub = unsafe { std::mem::transmute::<usize, fn(usize, &T)>(sub) };
            sub(slf, e);
        }
    }

    pub fn sub(&mut self, f: fn(&T)) {
        // NOTE: if fn is not subbed but this error still generates
        //       try #[inline(never)] and #[no_mangle] for the fn
        debug_assert!(!self.subbed_fns.contains(&f), "fn is already subbed");
        self.subbed_fns.push(f);
    }

    pub fn unsub(&mut self, f: fn(&T)) {
        let sub_idx = self
            .subbed_fns
            .iter()
            .position(|&s| std::ptr::fn_addr_eq(s, f))
            .unwrap_or_else(|| panic!("fn not subscribed"));
        self.subbed_fns.swap_remove(sub_idx);
    }

    pub fn sub_method<U>(&mut self, slf: &U, f: fn(&U, &T)) {
        let slf_addr = slf as *const _ as usize;
        let fn_addr = f as usize;
        // NOTE: if fn is not subbed but this error still generates
        //       try #[inline(never)] and #[no_mangle] for the fn
        debug_assert!(
            !self.subbed_methods.contains(&(slf_addr, fn_addr)),
            "fn is already subbed"
        );
        self.subbed_methods.push((slf_addr, fn_addr));
    }

    pub fn sub_method_mut<U>(&mut self, slf: &mut U, f: fn(&mut U, &T)) {
        let slf_addr = slf as *const _ as usize;
        let fn_addr = f as usize;
        // NOTE: if fn is not subbed but this error still generates
        //       try #[inline(never)] and #[no_mangle] for the fn
        debug_assert!(
            !self.subbed_methods.contains(&(slf_addr, fn_addr)),
            "fn is already subbed"
        );
        self.subbed_methods.push((slf_addr, fn_addr));
    }

    pub fn unsub_method<U, V>(&mut self, slf: &U, f: fn(V, &T)) {
        let slf_addr = slf as *const _ as usize;
        let fn_addr = f as usize;
        let sub_idx = self
            .subbed_methods
            .iter()
            .position(|&(s, f)| s == slf_addr && f == fn_addr)
            .unwrap_or_else(|| panic!("fn not subscribed"));
        self.subbed_methods.swap_remove(sub_idx);
    }
}

#[macro_export]
macro_rules! event {
    ($name: ident, $($member: ident: $member_ty: ty),*) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $member: $member_ty),*
        }
        impl Event for $name {}
        impl $name {
            #[allow(dead_code)]
            pub fn new($($member: $member_ty),*) -> Self {
                Self {
                    $($member),*
                }
            }
        }
    };
}

pub trait Event {}

event!(WindowResize, width: u32, height: u32);
