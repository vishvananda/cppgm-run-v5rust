struct input {};
struct public_ {};

template<class Chain, class Access, class Mode>
struct chainbuf;

template<class Chain, class Mode, class Access>
struct chainbuf {
  struct sentry {
    sentry(chainbuf<Chain, Mode, Access>* buf) : buf_(buf) {}
    chainbuf<Chain, Mode, Access>* buf_;
  };

  int underflow() {
    sentry t(this);
    return 0;
  }
};

int main() {
  chainbuf<int, input, public_> buffer;
  return buffer.underflow();
}
