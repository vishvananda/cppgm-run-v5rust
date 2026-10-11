namespace lib {
struct node {};
}

struct wrapper {
  struct node : lib::node {
    node *self() { return this; }
  };
};

int main() {
  wrapper::node value;
  wrapper::node *ptr = value.self();
  return ptr == &value ? 0 : 1;
}
