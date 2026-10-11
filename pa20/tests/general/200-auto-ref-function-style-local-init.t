struct Facet {
  int value;
};

Facet global;

Facet& use_facet() {
  return global;
}

int main() {
  global.value = 5;
  const auto& f(use_facet());
  return f.value == 5 ? 0 : 1;
}
