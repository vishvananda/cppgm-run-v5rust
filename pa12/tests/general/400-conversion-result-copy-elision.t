typedef unsigned long size_type;
void* operator new(size_type, void* p) noexcept { return p; }
int copies;
struct result {
  int value;
  result(int x) : value(x) {}
  result(const result& x) : value(x.value) { ++copies; }
};
struct source { operator result() { return result(7); } };
union storage { long long align; unsigned char bytes[sizeof(result)]; };
int main() {
  storage memory; source input;
  result* output = ::new(memory.bytes) result(input);
  return copies || output->value != 7;
}
