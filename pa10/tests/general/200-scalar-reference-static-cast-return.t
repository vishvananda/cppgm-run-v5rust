double read_value(double *result) {
  return static_cast<double &>(*result);
}

int main() {
  double d = 3.5;
  double x = read_value(&d);
  return x == 3.5 ? 0 : 1;
}
