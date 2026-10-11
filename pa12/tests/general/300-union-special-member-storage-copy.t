union Payload {
  int i;
  long long bits;
};

Payload make() {
  Payload p = {};
  p.bits = 7;
  return p;
}

int main() {
  Payload a = {};
  a = make();
  Payload b(a);
  return b.bits == 7 ? 0 : 1;
}
