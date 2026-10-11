namespace std {
template<class T>
T next(T value) { return value; }
}

template<class T>
struct direct_streambuf {
  long* begin;
  long* end;

  int seek_impl(long off) {
    using namespace std;
    long next = off;
    if(next < 0 || next > (end - begin)) {
      return 1;
    }
    return 0;
  }
};

int main() {
  direct_streambuf<int> buffer;
  long values[2] = {0, 0};
  buffer.begin = values;
  buffer.end = values + 2;
  return buffer.seek_impl(1);
}
