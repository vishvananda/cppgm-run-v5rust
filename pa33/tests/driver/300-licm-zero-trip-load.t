// N3485 1.9p1, 5.3.1p1: a loop that runs zero times does not dereference p, so
// moving the load out of the loop may not dereference a null p either
__attribute__((noinline)) int first_or(const int* p, int n, int r)
{
  for (int i = 0; i < n; ++i) r += *p + i;
  return r;
}
int main() { return first_or(nullptr, 0, 7) == 7 ? 0 : 1; }
