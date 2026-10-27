namespace N { int value = 7; }
namespace M { int value = 3; }
template<class T> int nested() {
  namespace A = N;
  int result = A::value;
  { namespace A = M; result += A::value; }
  return result + A::value + sizeof(T);
}
template<class T> int local_class() {
  struct Local {
    int get() { namespace A = N; return A::value + sizeof(T); }
  };
  Local object;
  return object.get();
}
template<class T> struct Box {
  int get() { namespace A = N; return A::value + sizeof(T); }
  int unused() { typename T::missing object; return sizeof(object); }
};
int main() {
  Box<int> x;
  Box<char> y;
  return nested<int>() + nested<char>() + local_class<int>()
       + local_class<char>() + x.get() + y.get() - 77;
}
