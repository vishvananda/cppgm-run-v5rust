void precedence(int x, int y, int z, int w) {
  x + y * z + w;
  (x + y) * (z + w);
  x * y << z;
  x + y > x * z;
  x <= y == z;
  x ^ y & z | w;
  x ^ y && z || w;
}
