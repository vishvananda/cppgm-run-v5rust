struct EndNode {};

struct Node : EndNode {};

struct Iter {
  EndNode *ptr;

  explicit Iter(Node *node) : ptr(node) {}
};

int main() {
  Node node;
  Iter iter(&node);
  (void)iter;
  return 0;
}
