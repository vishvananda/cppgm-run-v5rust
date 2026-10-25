// N3485 5.3.4p7: the invalid-length branch shares the completed
// constructor subobjects' unwind chain; expect exit 0
int live;
struct Marker {
  Marker() { ++live; }
  ~Marker() noexcept { --live; }
};
struct Buffer {
  Marker marker;
  char* bytes;
  Buffer(long count) : marker(), bytes(new char[count]) {}
  ~Buffer() { delete[] bytes; }
};
int main() {
  { Buffer buffer(3); buffer.bytes[2] = 7; }
  return live;
}
