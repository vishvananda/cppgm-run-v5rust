// N3485 focus: 12.3.2 [class.conv.fct], 13.3.1 [over.match.funcs]
struct D {};
struct S {
  operator D() &;
  operator D() && = delete;
};
void use(D);
void test(S& value) { use(value); }
