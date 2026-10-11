struct Node {
};

struct AllocBase {
};

struct Alloc : AllocBase {
};

struct Owner {
};

template<class T>
int consume_one(T * ptr) {
  return ptr->missing;
}

struct Scoped {
  int value;

  template<class... Args>
  Scoped(Owner * owner, Args... args)
    : value(consume_one(args...))
  {
    (void)owner;
  }

  Scoped(Node * node, AllocBase * alloc)
    : value(7)
  {
    (void)node;
    (void)alloc;
  }
};

int main() {
  Node node;
  Alloc alloc;
  Scoped scoped{&node, &alloc};
  return scoped.value == 7 ? 0 : 1;
}
