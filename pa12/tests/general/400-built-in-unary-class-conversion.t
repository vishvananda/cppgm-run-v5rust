class scalar_box {
public:
  scalar_box() : value_(0) {}
  scalar_box(long value) : value_(value) {}

  scalar_box &operator=(long value) {
    value_ = value;
    return *this;
  }

  operator long() const {
    return value_;
  }

  long value() const {
    return value_;
  }

private:
  long value_;
};

int main() {
  scalar_box big(12345);
  scalar_box result;
  result = -big;

  long inverted = ~scalar_box(0);
  return result.value() == -12345 && inverted == -1 ? 0 : 1;
}
