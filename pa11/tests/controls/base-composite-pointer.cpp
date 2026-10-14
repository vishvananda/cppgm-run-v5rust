// expect exit 0
// Related pointers share the qualified base type in comparisons and conditionals.
struct Base { int value; };
struct Derived : Base { int extra; };
Base* choose(bool first, Derived* derived, Base* base) { return first ? derived : base; }
Base* choose_reverse(bool first, Base* base, Derived* derived) { return first ? base : derived; }
const volatile Base* qualified(bool first, const Derived* derived, volatile Base* base)
{
  return first ? derived : base;
}
Base* right_operand(Base* base) { base->value = 7; return base; }
bool compare(Derived* derived, Base* base) { return derived == right_operand(base); }
bool compare_reverse(Base* base, Derived* derived) { return base == derived; }
bool ordered(Derived* derived, Base* base) { return derived <= base; }
int main()
{
  Derived object;
  Base other;
  Base* base = &object;
  other.value = 11;
  if (choose(true, &object, &other) != base) return 1;
  if (choose(false, &object, &other) != &other) return 2;
  if (choose_reverse(false, &other, &object) != base) return 3;
  if (choose(true, 0, &other) != 0) return 4;
  if (qualified(true, &object, &other) != base) return 5;
  if (!compare(&object, base) || object.value != 7) return 6;
  if (!compare_reverse(base, &object) || !ordered(&object, base)) return 7;
  if (!compare_reverse(0, 0)) return 8;
  return 0;
}
