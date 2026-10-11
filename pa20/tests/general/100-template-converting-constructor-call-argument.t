// HHC-285
struct milli {};
struct nano {};

template<class Rep, class Period>
struct duration {
  duration() {}

  template<class Rep2, class Period2>
  duration(const duration<Rep2, Period2> &) {}
};

typedef duration<long long, milli> milliseconds;
typedef duration<long long, nano> nanoseconds;

void sleep_for(const nanoseconds &) {}

int main() {
  sleep_for(milliseconds());
}
