extern const char symbol[];
const char symbol[2] = {'x', '\0'};

int main() {
  return symbol[0] == 'x' ? 0 : 1;
}
