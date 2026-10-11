// Each unit expands these inline functions (always_inline), so the closure
// types of their lambdas are defined in every unit: the call operator, the
// static invoker, the static variable in the lambda's body and a template
// specialized on the closure type are one entity across the units only when
// every unit emits them under one weak name.
template<class F> int* closure_tag(F) { static int tag = 0; return &tag; }

__attribute__((always_inline)) inline int next_ticket()
{
  auto issue = [] { static int ticket = 0; return ++ticket; };
  return issue();
}

__attribute__((always_inline)) inline int* ticket_tag()
{
  auto issue = [] { return 7; };
  return closure_tag(issue);
}

__attribute__((always_inline)) inline int (*ticket_source())()
{
  auto issue = [] { static int ticket = 100; return ++ticket; };
  return issue;
}
