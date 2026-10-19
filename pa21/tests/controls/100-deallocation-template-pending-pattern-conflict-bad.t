// The pattern's nonliteral noexcept operand waits for the specialization;
// an operand-free redeclaration still compares the pattern's stored value.
// expect reject
template<class T> struct Release {
  static void operator delete(void*) noexcept(sizeof(T) == 4);
};
template<class T> void Release<T>::operator delete(void*) noexcept {}
int main() { try { throw 1; } catch (int) {} }
