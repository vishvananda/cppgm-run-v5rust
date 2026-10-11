// An unused hosted inline wrapper may defer a reserved compiler intrinsic,
// but an ordinary lookup failure in its body is still a required diagnostic.
extern __inline __attribute__((__gnu_inline__)) int broken_wrapper()
{
  return nonexistent_identifier;
}

int main() { return 0; }
