struct Outer { struct Inner; };
class __attribute__((__visibility__("default"))) Outer::Inner {};

int main() {
  return 0;
}
