use crate::ctx::{Ctx, VarId};
use crate::var::Uniq;
use std::fmt;
use std::fmt::Write;

pub fn comma_sep(s: &str) -> String {
    let s_len = s.len();
    let mut ret = String::with_capacity(s_len + s_len / 3);

    let mut count = s_len % 3;
    if count == 0 {
        count = 3;
    }

    for c in s.chars() {
        if count == 0 {
            ret.push(',');
            count = 3;
        }
        ret.push(c);
        count -= 1;
    }

    ret
}

// Copied from take_mut crate
pub fn take<T, F>(mut_ref: &mut T, closure: F)
where
    F: FnOnce(T) -> T,
{
    use std::panic;
    use std::ptr;

    unsafe {
        let old_t = ptr::read(mut_ref);
        let new_t = panic::catch_unwind(panic::AssertUnwindSafe(|| closure(old_t)))
            .unwrap_or_else(|_| ::std::process::abort());
        ptr::write(mut_ref, new_t);
    }
}

#[allow(dead_code)]
pub fn show_arg_list(ctx: &Ctx, args: &[VarId]) -> String {
    let mut s = String::new();
    s.push('[');
    let mut add_comma = false;
    for arg in args {
        if add_comma {
            s.push_str(", ");
        } else {
            add_comma = true;
        }
        write!(s, "{}", ctx.get_var(*arg)).unwrap();
    }
    s.push(']');
    s
}

static BASE62_CHARS: &'static [u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub fn base62_encode(uniq: Uniq, w: &mut dyn Write) -> fmt::Result {
    let uniq = uniq.0.get();

    let mut i = uniq % 62;
    let mut r = uniq / 62;

    while i != 0 {
        w.write_char(char::from(BASE62_CHARS[i as usize]))?;
        i = r % 62;
        r /= 62;
    }

    Ok(())
}
