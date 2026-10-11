void statements(bool x, bool y, int (&values)[3]) {
  if (x)
    if (y)
      f();
    else
      g();
  for (auto value : values)
    use(value);
}
