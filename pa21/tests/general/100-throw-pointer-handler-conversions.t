// N3485 15.3p3, 4.4, 4.10: a pointer handler matches a thrown pointer
// convertible by a qualification conversion or by conversion to a public
// base (first level only), and std::nullptr_t. A privately derived class has no public base, so its
// type_info must say so (Itanium 2.9.5: __vmi_class_type_info, not
// __si_class_type_info). Each test returns 0 when it matches as g++ does.
struct Base { int b; Base() : b(1) {} virtual ~Base() {} };
struct Derived : Base { int d; Derived() : d(3) {} };
struct Private : private Base {};
static Derived derived;
static Private privately;
static char text[] = "x";

int qualification() {
  try { throw text + 0; } catch (const char* p) { return p == text ? 0 : 1; }
  return 1;
}
int multilevel() {
  char* p = text;
  try { throw &p; } catch (const char* const* q) { return *q == text ? 0 : 1; }
  return 1;
}
int unsafe_multilevel() {
  char* p = text;
  try { try { throw &p; } catch (const char**) { return 1; } }
  catch (char** q) { return *q == text ? 0 : 1; }
  return 1;
}
int to_base() {
  try { throw &derived; } catch (const Base* p) { return p == &derived && p->b == 1 ? 0 : 1; }
  return 1;
}
int null_to_base() {
  Derived* none = 0;
  try { throw none; } catch (Base* p) { return p == 0 ? 0 : 1; }
  return 1;
}
int private_base() {
  try { try { throw &privately; } catch (Base*) { return 1; } } catch (Private*) { return 0; }
  return 1;
}
int no_base_at_second_level() {
  Derived* p = &derived;
  try { try { throw &p; } catch (Base**) { return 1; } } catch (Derived**) { return 0; }
  return 1;
}
int null_pointer() {
  try { throw nullptr; } catch (Base* p) { return p == 0 ? 0 : 1; }
  return 1;
}
int main() {
  return qualification() + multilevel() + unsafe_multilevel() + to_base() +
    null_to_base() + private_base() + no_base_at_second_level() + null_pointer();
}
