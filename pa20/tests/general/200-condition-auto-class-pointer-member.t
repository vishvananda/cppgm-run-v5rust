struct Node {
  int hash_code;
};

Node global_node = {7};

Node * locate() {
  return &global_node;
}

int main() {
  if (auto node = locate()) {
    return node->hash_code == 7 ? 0 : 1;
  }
  return 2;
}
