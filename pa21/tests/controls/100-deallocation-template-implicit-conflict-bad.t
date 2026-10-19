// An ordinary class-template member retains its pattern exception facts.
// An omitted deallocation specification is non-throwing on redeclaration.
// expect reject
template<class T> struct Release {
  static void operator delete[](void*) noexcept(false);
};
template<class T> void Release<T>::operator delete[](void*) {}
int main() { try { throw 1; } catch (int) {} }
