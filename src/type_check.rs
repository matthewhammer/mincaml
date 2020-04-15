/*

Unification
-----------

Find substitutions that make two types the same.

 */

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::parser::Expr;

/// Type variables are represented as unique integers.
pub type TyVar = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unit,
    Bool,
    Int,
    Float,
    Fun { args: Vec<Type>, ret: Box<Type> },
    Tuple(Vec<Type>),
    Array(Box<Type>),
    Var(TyVar),
}

/// Create initial type environment with built-is stuff.
fn mk_type_env() -> HashMap<String, Type> {
    let mut env = HashMap::new();
    env.insert(
        "print_int".to_owned(),
        Type::Fun {
            args: vec![Type::Int],
            ret: Box::new(Type::Unit),
        },
    );
    env
}

fn new_tyvar(tyvar_cnt: &mut u64) -> Type {
    let tyvar = *tyvar_cnt;
    *tyvar_cnt = *tyvar_cnt + 1;
    Type::Var(tyvar)
}

#[derive(Debug)]
pub enum TypeErr {
    /// Can't unify these two types
    UnifyError(Type, Type),
    /// Unbound variable
    UnboundVar(String),
}

pub fn type_check(expr: &Expr) -> Result<Type, TypeErr> {
    let mut tyvar_cnt = 0;
    let mut env = mk_type_env();
    let mut substs = HashMap::new();
    type_check_(&mut tyvar_cnt, &mut substs, &mut env, expr)
}

fn type_check_(
    tyvar_cnt: &mut u64,
    substs: &mut HashMap<TyVar, Type>,
    env: &mut HashMap<String, Type>,
    expr: &Expr,
) -> Result<Type, TypeErr> {
    match expr {
        Expr::Unit => Ok(Type::Unit),
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::Int(_) => Ok(Type::Int),
        Expr::Float(_) => Ok(Type::Float),
        Expr::Not(e) => {
            let e_ty = type_check_(tyvar_cnt, substs, env, e)?;
            unify(substs, &Type::Bool, &e_ty)?;
            Ok(Type::Bool)
        }
        Expr::Neg(e) => {
            let e_ty = type_check_(tyvar_cnt, substs, env, e)?;
            unify(substs, &Type::Int, &e_ty)?;
            Ok(Type::Int)
        }
        Expr::Add(e1, e2) | Expr::Sub(e1, e2) => {
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            unify(substs, &Type::Int, &e1_ty)?;
            unify(substs, &Type::Int, &e2_ty)?;
            Ok(Type::Int)
        }
        Expr::FNeg(e) => {
            let e_ty = type_check_(tyvar_cnt, substs, env, e)?;
            unify(substs, &Type::Float, &e_ty)?;
            Ok(Type::Float)
        }
        Expr::FAdd(e1, e2) | Expr::FSub(e1, e2) | Expr::FMul(e1, e2) | Expr::FDiv(e1, e2) => {
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            unify(substs, &Type::Float, &e1_ty)?;
            unify(substs, &Type::Float, &e2_ty)?;
            Ok(Type::Float)
        }
        Expr::Eq(e1, e2) | Expr::Le(e1, e2) => {
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            unify(substs, &e1_ty, &e2_ty)?;
            Ok(Type::Bool)
        }
        Expr::If(e1, e2, e3) => {
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            let e3_ty = type_check_(tyvar_cnt, substs, env, e3)?;
            unify(substs, &e1_ty, &Type::Bool)?;
            unify(substs, &e2_ty, &e3_ty)?;
            Ok(e2_ty)
        }
        Expr::Let {
            ref id,
            ref rhs,
            body,
        } => {
            let bndr_type = new_tyvar(tyvar_cnt);
            let rhs_type = type_check_(tyvar_cnt, substs, env, rhs)?;
            unify(substs, &bndr_type, &rhs_type)?;
            // FIXME: string clone
            env.insert(id.clone(), bndr_type);
            let ret = type_check_(tyvar_cnt, substs, env, body);
            env.remove(id);
            ret
        }
        Expr::Var(var) => match env.get(var) {
            Some(ty) => Ok(ty.clone()),
            None => Err(TypeErr::UnboundVar(var.clone())),
        },
        Expr::LetRec {
            name,
            args,
            rhs,
            body,
        } => {
            // Type variables for the arguments
            let mut arg_tys: Vec<Type> = Vec::with_capacity(args.len());
            for _ in args {
                arg_tys.push(new_tyvar(tyvar_cnt));
            }
            // Type variable for the RHS
            let rhs_ty = new_tyvar(tyvar_cnt);
            // We can now give type to the recursive function
            let fun_ty = Type::Fun {
                args: arg_tys.clone(),
                ret: Box::new(rhs_ty.clone()),
            };
            // RHS and body will be type checked with `name` and args in scope
            env.insert(name.clone(), fun_ty.clone());
            for (arg, arg_ty) in args.iter().zip(arg_tys.iter()) {
                env.insert(arg.clone(), arg_ty.clone());
            }
            // Type check RHS
            let rhs_ty_ = type_check_(tyvar_cnt, substs, env, rhs)?;
            unify(substs, &rhs_ty, &rhs_ty_)?;
            // Type check body
            let ret = type_check_(tyvar_cnt, substs, env, body);
            // Reset environment
            env.remove(name);
            for arg in args.iter() {
                env.remove(arg);
            }
            ret
        }
        Expr::App { fun, args } => {
            let ret_ty = new_tyvar(tyvar_cnt);
            let mut arg_tys: Vec<Type> = Vec::with_capacity(args.len());
            for arg in args {
                arg_tys.push(type_check_(tyvar_cnt, substs, env, arg)?);
            }
            let fun_ty = Type::Fun {
                args: arg_tys,
                ret: Box::new(ret_ty.clone()),
            };
            let fun_ty_ = type_check_(tyvar_cnt, substs, env, fun)?;
            unify(substs, &fun_ty, &fun_ty_)?;
            Ok(ret_ty)
        }
        Expr::Tuple(args) => {
            let mut arg_tys: Vec<Type> = Vec::with_capacity(args.len());
            for arg in args {
                arg_tys.push(type_check_(tyvar_cnt, substs, env, arg)?);
            }
            Ok(Type::Tuple(arg_tys))
        }
        Expr::LetTuple { bndrs, rhs, body } => {
            let mut bndr_tys: Vec<Type> = Vec::with_capacity(bndrs.len());
            for _ in bndrs {
                bndr_tys.push(new_tyvar(tyvar_cnt));
            }
            let tuple_ty = Type::Tuple(bndr_tys.clone());
            let rhs_ty = type_check_(tyvar_cnt, substs, env, rhs)?;
            unify(substs, &rhs_ty, &tuple_ty)?;
            for (bndr, bndr_type) in bndrs.iter().zip(bndr_tys.into_iter()) {
                env.insert(bndr.clone(), bndr_type);
            }
            let ret = type_check_(tyvar_cnt, substs, env, body);
            for bndr in bndrs.iter() {
                env.remove(bndr);
            }
            ret
        }
        Expr::Array(e1, e2) => {
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            unify(substs, &e1_ty, &Type::Int)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            Ok(Type::Array(Box::new(e2_ty)))
        }
        Expr::Get(e1, e2) => {
            let array_elem_ty = new_tyvar(tyvar_cnt);
            let array_ty = Type::Array(Box::new(array_elem_ty.clone()));
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            unify(substs, &e1_ty, &array_ty)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            unify(substs, &e2_ty, &Type::Int)?;
            Ok(array_elem_ty)
        }
        Expr::Put(e1, e2, e3) => {
            let array_elem_ty = new_tyvar(tyvar_cnt);
            let array_ty = Type::Array(Box::new(array_elem_ty.clone()));
            let e1_ty = type_check_(tyvar_cnt, substs, env, e1)?;
            unify(substs, &e1_ty, &array_ty)?;
            let e2_ty = type_check_(tyvar_cnt, substs, env, e2)?;
            unify(substs, &e2_ty, &Type::Int)?;
            let e3_ty = type_check_(tyvar_cnt, substs, env, e3)?;
            unify(substs, &e3_ty, &array_elem_ty)?;
            Ok(Type::Unit)
        }
    }
}

fn deref_tyvar(substs: &mut HashMap<TyVar, Type>, mut tyvar: TyVar) -> Type {
    loop {
        match substs.get(&tyvar) {
            None => {
                return Type::Var(tyvar);
            }
            Some(Type::Var(tyvar_)) => {
                tyvar = *tyvar_;
            }
            Some(other) => {
                return other.clone();
            }
        }
    }
}

fn norm_ty<'a>(subst: &'a HashMap<TyVar, Type>, mut ty: &'a Type) -> &'a Type {
    loop {
        match ty {
            Type::Var(tyvar) => match subst.get(tyvar) {
                None => {
                    return ty;
                }
                Some(ty_) => {
                    ty = ty_;
                }
            },
            _ => {
                return ty;
            }
        }
    }
}

fn unify(substs: &mut HashMap<TyVar, Type>, ty1: &Type, ty2: &Type) -> Result<(), TypeErr> {
    let ty1 = norm_ty(substs, ty1).clone();
    let ty2 = norm_ty(substs, ty2).clone();
    match (&ty1, &ty2) {
        (Type::Unit, Type::Unit)
        | (Type::Bool, Type::Bool)
        | (Type::Int, Type::Int)
        | (Type::Float, Type::Float) => Ok(()),
        (
            Type::Fun {
                args: args1,
                ret: ret1,
            },
            Type::Fun {
                args: args2,
                ret: ret2,
            },
        ) => {
            if args1.len() != args2.len() {
                return Err(TypeErr::UnifyError(ty1.clone(), ty2.clone()));
            }
            for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                unify(substs, arg1, arg2)?;
            }
            unify(substs, &*ret1, &*ret2)
        }

        (Type::Var(var), ty) | (ty, Type::Var(var)) => {
            // TODO occurs check
            substs.insert(*var, ty.clone());
            Ok(())
        }

        (Type::Tuple(args1), Type::Tuple(args2)) => {
            if args1.len() != args2.len() {
                return Err(TypeErr::UnifyError(ty1.clone(), ty2.clone()));
            }
            for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                unify(substs, arg1, arg2)?;
            }
            Ok(())
        }
        (Type::Array(ty1), Type::Array(ty2)) => unify(substs, ty1, ty2),
        _ => Err(TypeErr::UnifyError(ty1.clone(), ty2.clone())),
    }
}

#[test]
fn unify_test_1() {
    let mut tyvar_cnt = 0;
    let mut substs = HashMap::new();

    let ty1 = Type::Int;
    let ty2 = new_tyvar(&mut tyvar_cnt);
    unify(&mut substs, &ty1, &ty2).unwrap();
    assert_eq!(norm_ty(&substs, &ty2), &Type::Int);
    assert_eq!(norm_ty(&substs, &ty1), &Type::Int);

    let ty3 = new_tyvar(&mut tyvar_cnt);
    unify(&mut substs, &ty2, &ty3).unwrap();
    assert_eq!(norm_ty(&substs, &ty2), &Type::Int);
    assert_eq!(norm_ty(&substs, &ty3), &Type::Int);
}

#[test]
fn unify_test_2() {
    let mut tyvar_cnt = 0;
    let mut substs = HashMap::new();

    let ty1 = Type::Int;
    let ty2 = new_tyvar(&mut tyvar_cnt);
    let ty3 = new_tyvar(&mut tyvar_cnt);
    let ty4 = new_tyvar(&mut tyvar_cnt);
    let ty5 = new_tyvar(&mut tyvar_cnt);

    unify(&mut substs, &ty2, &ty3).unwrap();
    unify(&mut substs, &ty2, &ty4).unwrap();
    unify(&mut substs, &ty2, &ty5).unwrap();
    unify(&mut substs, &ty5, &ty1).unwrap();

    assert_eq!(norm_ty(&substs, &ty1), &Type::Int);
    assert_eq!(norm_ty(&substs, &ty2), &Type::Int);
    assert_eq!(norm_ty(&substs, &ty3), &Type::Int);
    assert_eq!(norm_ty(&substs, &ty4), &Type::Int);
    assert_eq!(norm_ty(&substs, &ty5), &Type::Int);
}