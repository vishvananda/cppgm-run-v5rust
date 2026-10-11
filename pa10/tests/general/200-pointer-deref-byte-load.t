static char data[] = { '1', '\n', 0 };
static char* p = data;
char take() { return *p++; }
char peek() { return *p; }
int main() {
  return take() == '1' && peek() == '\n' ? 0 : 1;
}
