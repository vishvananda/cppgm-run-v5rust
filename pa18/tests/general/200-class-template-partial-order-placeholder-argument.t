// VALIDATION: compile-pass
// A partial-order transformed parameter can put internal placeholder types
// inside a class-template-id. Deduction must use the placeholder type carried
// by the transformed argument, not try to resolve its diagnostic text.

struct na {};

template<class TA, class TB, class Info, bool ForceMutable>
struct mutant_relation {};

template<class TA, class TB, class Info, bool ForceMutable>
char select(mutant_relation<TA, TB, Info, ForceMutable> *);

template<class TA, class TB, bool ForceMutable>
int select(mutant_relation<TA, TB, na, ForceMutable> *);

using relation_type = mutant_relation<int, long, na, false>;

int main()
{
  return sizeof(select(static_cast<relation_type *>(0))) == sizeof(int) ? 0 : 1;
}
