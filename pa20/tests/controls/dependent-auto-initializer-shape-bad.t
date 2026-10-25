// expect reject
struct Source { int operator()() const { return 7; } };
template<class T> int read(T source) {
  if (auto* pointer = source()) return *pointer;
  return 0;
}
int main() { return read(Source()); }
