extern "C" {
int compare_f(int (*cmp)(const void *, const void *)
              __attribute__((__noescape__)));
}

int main() {
  return 0;
}
