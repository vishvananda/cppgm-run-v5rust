template<class T>
struct Vec {
  Vec() {}
};

struct A {};
struct B {};

struct Scope {
  Scope* parent;
};

struct QualifiedName {
  bool rooted;
  const char* name;
  const char* qualifier;
};

namespace cpp_scope_lookup {

template<typename Result,
         typename ScopeT,
         typename ResolveUnqualifiedQualifier,
         typename ResolveDirectQualifier,
         typename FinalLookup>
Result lookup_qualified(ScopeT & root,
                        const QualifiedName & qualified,
                        const ResolveUnqualifiedQualifier & resolve_unqualified_qualifier,
                        const ResolveDirectQualifier & resolve_direct_qualifier,
                        const FinalLookup & final_lookup)
{
  if(qualified.rooted && !qualified.qualifier) {
    return final_lookup(root, qualified.name);
  }

  if(!qualified.rooted && !qualified.qualifier) {
    return Result();
  }

  ScopeT * current = 0;
  if(qualified.rooted) {
    current = &root;
  } else {
    current = resolve_unqualified_qualifier(qualified.qualifier);
    if(!current) {
      return Result();
    }
  }

  current = resolve_direct_qualifier(*current, qualified.qualifier);
  if(!current) {
    return Result();
  }

  return final_lookup(*current, qualified.name);
}

}  // namespace cpp_scope_lookup

struct Analyzer {
  Scope* root;

  template<typename Result, typename FinalLookup>
  Result lookup_qualified_generic(Scope & scope,
                                  const QualifiedName & qualified,
                                  const FinalLookup & final_lookup)
  {
    return cpp_scope_lookup::lookup_qualified<Result>(
        *root, qualified,
        [this, &scope](const char *) -> Scope *
        {
          return &scope;
        },
        [](Scope & target, const char *) -> Scope *
        {
          return &target;
        },
        final_lookup);
  }

  Vec<A*> lookup_a(Scope & scope, const QualifiedName & qualified)
  {
    return lookup_qualified_generic<Vec<A*> >(
        scope, qualified,
        [](Scope &, const char *) -> Vec<A*>
        {
          return Vec<A*>();
        });
  }

  Vec<B*> lookup_b(Scope & scope, const QualifiedName & qualified)
  {
    return lookup_qualified_generic<Vec<B*> >(
        scope, qualified,
        [](Scope &, const char *) -> Vec<B*>
        {
          return Vec<B*>();
        });
  }
};

int main()
{
  Scope root = {0};
  QualifiedName q;
  q.rooted = true;
  q.name = "x";
  q.qualifier = 0;

  Analyzer a;
  a.root = &root;
  a.lookup_a(root, q);
  a.lookup_b(root, q);
  return 0;
}
