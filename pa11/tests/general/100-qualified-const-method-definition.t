struct X {
  unsigned int flags() const;
  unsigned int v;
};

inline unsigned int X::flags() const {
  return v;
}

int main() {
  X x = {1};
  return x.flags() == 1 ? 0 : 1;
}
