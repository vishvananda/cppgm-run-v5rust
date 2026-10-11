// HHC-054: intended outcome: should parse successfully as a real class-specifier.
template<class T>
struct alignas(__alignof(T)) X {};

int main() {
  return 0;
}
