struct Scope {};

template <class T>
struct Box {
  Box() {}
};

namespace semantic_lookup {

Box<int *> lookup_direct_function_templates(Scope &, const char *)
{
  return Box<int *>();
}

}  // namespace semantic_lookup

struct Analyzer {
  template <class Result, class FinalLookup>
  Result lookup_qualified_class_or_namespace_generic(Scope & scope,
                                                     const char * qualified,
                                                     const FinalLookup & final_lookup)
  {
    return final_lookup(scope, qualified);
  }

  Box<int *> lookup_functions(Scope & scope, const char * qualified)
  {
    if(qualified) {
      return lookup_qualified_class_or_namespace_generic<Box<int *> >(
          scope,
          qualified,
          [](Scope & target, const char * lookup_name) -> Box<int *>
          {
            return semantic_lookup::lookup_direct_function_templates(target, lookup_name);
          });
    }
    return Box<int *>();
  }
};

int main()
{
  Analyzer analyzer;
  Scope scope;
  analyzer.lookup_functions(scope, "x");
}
