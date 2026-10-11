struct S {
  int x;
};

S&& move_ref(S& s);

int read_move(S&& s) {
  return s.x;
}

int main() {
  S s;
  s.x = 11;
  return read_move(move_ref(s));
}
