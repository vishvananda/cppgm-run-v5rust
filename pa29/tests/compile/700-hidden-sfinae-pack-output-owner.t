typedef decltype(sizeof(0)) size_t;
void* operator new(size_t, void*) noexcept;

template<class T>
T&& declval();

namespace detail {

template<class T, class... Args,
         class = decltype(::new((void*)0) T(declval<Args>()...))>
T* hidden_construct_at(T* p, Args&&... args) {
  return p;
}

}

struct Node {
  Node() {}
};

Node* build_node(Node* p) {
  return detail::hidden_construct_at(p);
}
