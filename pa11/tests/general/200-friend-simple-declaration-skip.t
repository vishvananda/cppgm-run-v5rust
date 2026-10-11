struct X {
  friend void swap(X&, X&);
  friend X current();
};

int main() { return 0; }
