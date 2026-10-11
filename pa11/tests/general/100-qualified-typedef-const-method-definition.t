struct X {
  typedef unsigned int flags_t;
  flags_t flags() const;
  unsigned int v;
};

inline X::flags_t X::flags() const {
  return v;
}

int main() {
  X x = {1};
  return x.flags() == 1 ? 0 : 1;
}
