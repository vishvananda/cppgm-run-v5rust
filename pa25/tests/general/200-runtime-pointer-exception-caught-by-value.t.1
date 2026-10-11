// N3485 15.3p3, Itanium C++ ABI 2.5.3: a pointer exception is caught by
// value. A pointer handler receives the thrown pointer, a reference to a
// pointer binds to a copy of it, and rethrowing keeps the same pointer; a
// class handler still receives the exception object itself.
struct B { int v; };
static B object = {7};
static int number = 9;
static const char text[] = "hello";
int by_value() { try { throw &object; } catch (B* p) { return p == &object && p->v == 7 ? 0 : 1; } }
int by_const_ref() { try { throw text + 1; } catch (const char* const& p) { return p == text + 1 && *p == 'e' ? 0 : 2; } }
int scalar_pointer() { try { throw &number; } catch (int* p) { return *p == 9 ? 0 : 4; } }
int nested() {
  try { try { throw &number; } catch (int* p) { if (p != &number) return 8; throw; } }
  catch (int* q) { return q == &number ? 0 : 8; }
  return 8;
}
int class_still_works() { try { B thrown = {3}; throw thrown; } catch (B& b) { return b.v == 3 ? 0 : 16; } }
int main() { return by_value() + by_const_ref() + scalar_pointer() + nested() + class_still_works(); }
