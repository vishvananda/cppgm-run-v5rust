extern thread_local int value;
int read_first() { return value; }
thread_local int value = 3;
int read_second() { return value; }
namespace {
  extern thread_local const unsigned other;
  unsigned read_other() { return other; }
  thread_local const unsigned other = 4;
}
int main() { return read_first() + read_second() + read_other(); }
