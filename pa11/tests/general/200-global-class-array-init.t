struct Entry {
  const char *name;
  int value;

  Entry(const char *name_, int value_) : name(name_), value(value_) {}
};

Entry table[] = {{"a", 1}, {"b", 2}};

int main() {
  return table[0].value == 1 && table[1].value == 2 ? 0 : 1;
}
