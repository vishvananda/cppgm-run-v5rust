struct S {
  int x;
};

S&& move_ref(S& s);

int read_ref(const S& s) {
  return s.x;
}

int main() {
  S s;
  s.x = 7;
  return read_ref(move_ref(s));
}
