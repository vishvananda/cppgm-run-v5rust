namespace unsigned_empty_functional_cast_call_arg {

unsigned take(unsigned value, const char *) {
  return value;
}

}

int main() {
  return unsigned_empty_functional_cast_call_arg::take(unsigned(), "unsigned()");
}
