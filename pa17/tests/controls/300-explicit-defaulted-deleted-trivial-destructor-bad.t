// expect reject
// Semantic definition availability is satisfied, but emission of a trivial
// defaulted destructor still demands its unavailable subobject destruction.
struct deleted_destructor {
  ~deleted_destructor() = delete;
};
template<class T> struct box {
  T value;
  ~box() = default;
};
template box<deleted_destructor>::~box();
int main() { return 0; }
