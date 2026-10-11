// N3485 7.6.2p1, 7.6.2p4: an alignment-specifier on a block-scope object
// declaration applies to that object, so its automatic storage has the
// requested alignment, including alignments above the stack's own.
struct alignas(16) Vector { char lane; };

unsigned long misaligned(const void* address, unsigned long alignment)
{
  return reinterpret_cast<unsigned long>(address) & (alignment - 1);
}

int depth(int remaining)
{
  alignas(64) char line[3];
  alignas(double) char scalar;
  alignas(32) Vector vector;
  line[0] = scalar = vector.lane = 1;
  int result = (misaligned(line, 64) != 0) | (misaligned(&scalar, 8) != 0) << 1 |
    (misaligned(&vector, 32) != 0) << 2;
  return result | (remaining ? depth(remaining - 1) : 0);
}

int main()
{
  return depth(4);
}
