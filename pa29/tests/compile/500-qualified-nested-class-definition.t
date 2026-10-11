struct Outer { struct Inner; };
struct Outer::Inner {};

int main() {
  return 0;
}
