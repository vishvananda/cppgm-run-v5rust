// expect exit 0
// Deleted defaulted copies satisfy named roots and still emit synthesized code.
struct uncopyable {
  uncopyable(const uncopyable&) = delete;
};
template<class T> struct box {
  T value;
  box(const box&) = default;
  box(box&&) = default;
  ~box() = default;
};
template box<uncopyable>::box(const box<uncopyable>&);
template box<uncopyable>::box(box<uncopyable>&&);
int main() { return 0; }
