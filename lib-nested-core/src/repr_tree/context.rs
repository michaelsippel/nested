use {
    r3vi::{view::{OuterViewPort, singleton::*}, buffer::{singleton::*}},
    laddertypes::{TypeDict, TypeTerm, TypeID, MorphismType, MorphismBase, Morphism},
    crate::{
        repr_tree::{ReprTree, ReprTreeExt, GenericReprTreeMorphism},
        edit_tree::EditTree
    },
    std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    }
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

#[derive(Clone)]
pub struct Context {
    /// assigns a name to every type
    pub type_dict: Arc<RwLock<TypeDict>>,

    pub morphisms: laddertypes::morphism::MorphismBase< GenericReprTreeMorphism >,

    /// named vertices of the graph
    nodes: HashMap< String, Arc<RwLock<ReprTree>> >,

    /// todo: beautify
    /// types that can be edited as lists
    /// do we really need this?
    pub list_types: Vec< TypeID >,
    pub meta_chars: Vec< char >,

    edittree_hook: Arc< dyn Fn(&mut EditTree, TypeTerm) + Send +Sync +'static >,

    /// recursion
    parent: Option<Arc<RwLock<Context>>>,
}

impl Context {
    pub fn with_parent(
        parent: Option<Arc<RwLock<Context>>>
    ) -> Self {
        let mut dict = TypeDict::new();
        let list_typeid = dict.add_typename("List".into());

        Context {
            type_dict: match parent.as_ref() {
                Some(p) => p.read().unwrap().type_dict.clone(),
                None => Arc::new(RwLock::new(dict))
            },
            morphisms: MorphismBase::new( list_typeid ),
            nodes: HashMap::new(),
            list_types: match parent.as_ref() {
                Some(p) => p.read().unwrap().list_types.clone(),
                None => Vec::new()
            },
            meta_chars: match parent.as_ref() {
                Some(p) => p.read().unwrap().meta_chars.clone(),
                None => Vec::new()
            },
            parent,
            edittree_hook: Arc::new(|_et, _t| {})
        }
    }

    pub fn new() -> Self {
        Context::with_parent(None)
    }

    pub fn set_edittree_hook(&mut self, hook: Arc< dyn Fn(&mut EditTree, TypeTerm) + Send +Sync +'static >) {
        self.edittree_hook = hook;
    }

    pub fn depth(&self) -> usize {
        if let Some(parent) = self.parent.as_ref() {
            parent.read().unwrap().depth() + 1
        } else {
            0
        }
    }

    pub fn apply_morphism( &self, rt: &Arc<RwLock<ReprTree>>, ty: &MorphismType ) {
        if let Some(path)
            = self.morphisms.find_morphism_path( ty.clone().normalize() )
        {
            let mut path = path.into_iter();
            if let Some(mut src_type) = path.next() {
                for dst_type in path {
                   if let Some(( m, mut τ, σ )) =
                       self.morphisms.find_morphism_with_subtyping(
                        &laddertypes::MorphismType {
                            src_type: src_type.clone(),
                            dst_type: dst_type.clone()
                        }
                    ) {                   
                        let mut rt = rt.descend( τ ).expect("descend src repr");
                        (m.setup_projection)( &mut rt, &σ );
                    }

                    src_type = dst_type;
                }
            }
        } else {
            eprintln!("no path found");
        }    
    }

    pub fn make_repr(ctx: &Arc<RwLock<Self>>, t: &TypeTerm) -> Arc<RwLock<ReprTree>> {
        let rt = Arc::new(RwLock::new(ReprTree::new( TypeTerm::unit() )));
        ctx.read().unwrap().apply_morphism( &rt, &MorphismType{ src_type: TypeTerm::unit(), dst_type: t.clone() } );
        rt
    }

    pub fn parse(ctx: &Arc<RwLock<Self>>, s: &str) -> TypeTerm {
        ctx.read().unwrap().type_term_from_str(s).expect("could not parse type term")
    }

    pub fn add_typename(&mut self, tn: &str) -> TypeID {
        self.type_dict.write().unwrap().add_typename(tn.to_string())
    }

    pub fn add_varname(&mut self, vn: &str) -> TypeID {
        self.type_dict.write().unwrap().add_varname(vn.to_string())
    }

    pub fn add_synonym(&mut self, new: &str, old: &str) {
        self.type_dict.write().unwrap().add_synonym(new.to_string(), old.to_string());
    }

    pub fn add_list_typename(&mut self, tn: &str) {
        let tid = self.add_typename(tn);
        self.list_types.push( tid );
    }

    pub fn is_list_type(&self, t: &TypeTerm) -> bool {
        match t {
            TypeTerm::TypeID(id) => {
                self.list_types.contains(id)
            }
            TypeTerm::Ladder(args) |
            TypeTerm::App(args) => {
                if args.len() > 0 {
                    if self.is_list_type(&args[0]) {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false
        }
    }

    pub fn get_typeid(&self, tn: &str) -> Option<TypeID> {
        self.type_dict.read().unwrap().get_typeid(&tn.into())
    }

    pub fn get_fun_typeid(&self, tn: &str) -> Option<u64> {
        match self.get_typeid(tn) {
            Some(TypeID::Fun(x)) => Some(x),
            _ => None
        }
    }

    pub fn get_typename(&self, tid: &TypeID) -> Option<String> {
        self.type_dict.read().unwrap().get_typename(tid)
    }

    pub fn get_var_typeid(&self, tn: &str) -> Option<u64> {
        match self.get_typeid(tn) {
            Some(TypeID::Var(x)) => Some(x),
            _ => None
        }
    }

    pub fn type_term_from_str(&self, tn: &str) -> Result<TypeTerm, laddertypes::parser::ParseError> {
        self.type_dict.write().unwrap().parse(&tn)
    }

    pub fn type_term_to_str(&self, t: &TypeTerm) -> String {
        self.type_dict.read().unwrap().unparse(&t)
    }

    /// adds an object without any representations
    pub fn add_obj(ctx: Arc<RwLock<Context>>, name: String, typename: &str) {
        let type_tag = ctx.read().unwrap()
            .type_dict.write().unwrap()
            .parse(typename).unwrap();
/*
        if let Some(node) = Context::make_node(&ctx, type_tag, SingletonBuffer::new(0).get_port()) {
            ctx.write().unwrap().nodes.insert(name, node);
        }
*/
    }

    pub fn get_obj(&self, name: &String) -> Option< Arc<RwLock<ReprTree>> > {
        if let Some(obj) = self.nodes.get(name) {
            Some(obj.clone())
        } else if let Some(parent) = self.parent.as_ref() {
            parent.read().unwrap().get_obj(name)
        } else {
            None
        }
    }

    pub fn setup_edittree(
        &self,
        rt: Arc<RwLock<ReprTree>>,
        depth: OuterViewPort<dyn SingletonView<Item = usize>>
    ) -> Option<SingletonBuffer<Arc<RwLock<EditTree>>>> {
        if let Some(new_edittree) =
            rt.descend(self.type_term_from_str("EditTree").unwrap())
        {
            let typ = rt.read().unwrap().get_type().clone();
            let buf = new_edittree.singleton_buffer::<Arc<RwLock<EditTree>>>();
            (*self.edittree_hook)(
                &mut *buf.get().write().unwrap(),
                typ
            );
            Some(buf)
        } else {
            eprintln!("cant find edit tree repr {} ~Ψ~ {}",
                self.type_term_to_str(rt.read().unwrap().get_halo_type()),
                self.type_term_to_str(rt.read().unwrap().get_type())
            );
            None
        }
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

