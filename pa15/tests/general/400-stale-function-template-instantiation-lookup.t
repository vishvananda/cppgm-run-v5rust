template <class T>
T* addrof(T& x) {
  return &x;
}

struct base {
  base* left;
};

template <class T>
struct node : base {
  T value;
};

template <class T>
base** descend(node<T>* nd) {
  (void)addrof(*nd);
  return addrof(nd->left);
}

int main() {
  node<int> n{};
  n.left = 0;
  return descend(&n) == &n.left ? 0 : 1;
}
