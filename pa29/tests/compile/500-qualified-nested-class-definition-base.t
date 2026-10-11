struct Base {};
struct Outer { struct Inner; };
struct Outer::Inner : Base {};

int main() {
  return 0;
}
