// N3485 15.4p15: omitted scalar, array, sized and placement deallocation
// redeclarations remain non-throwing through normal and failed construction.
// expect exit 0
typedef decltype(sizeof(0)) size_type;
void operator delete(void*);
void operator delete[](void*);
int released, destroyed;
struct Scalar {
  Scalar(int n) { if (n) throw n; }
  ~Scalar() { ++destroyed; }
  static void* operator new(size_type);
  static void operator delete(void*, size_type);
};
alignas(16) char scalar_storage[64];
void* Scalar::operator new(size_type) { return scalar_storage; }
void Scalar::operator delete(void*, size_type) { ++released; }
struct Array {
  Array() { if (++attempt == 3) throw attempt; }
  ~Array() { ++destroyed; }
  static int attempt;
  static void* operator new[](size_type);
  static void operator delete[](void*, size_type);
};
int Array::attempt;
alignas(16) char array_storage[64];
void* Array::operator new[](size_type) { return array_storage; }
void Array::operator delete[](void*, size_type) { ++released; }
template<class T> struct Placed {
  Placed(int n) { if (n) throw n; }
  static void* operator new(size_type, int);
  static void operator delete(void*, int) noexcept;
};
alignas(16) char placement_storage[64];
template<class T> void* Placed<T>::operator new(size_type, int) { return placement_storage; }
template<class T> void Placed<T>::operator delete(void*, int) { ++released; }
int main() {
  Scalar* p = new Scalar(0);
  delete p;
  try { new Scalar(1); } catch (int) {}
  try { new Array[4]; } catch (int) {}
  try { new (7) Placed<int>(2); } catch (int) {}
  return released == 4 && destroyed == 3 ? 0 : 1;
}
