struct copy_impl {
  template <class In, class Out>
  static Out operator()(In first, In last, Out result) {
    while (first != last) {
      *result = *first;
      ++first;
      ++result;
    }
    return result;
  }
};

template <class Algorithm, class In, class Out>
Out dispatch(In first, In last, Out result) {
  return Algorithm()(first, last, result);
}

int src[2] = {1, 2};
int dst[2];

int main() {
  dispatch<copy_impl>(src, src + 2, dst);
  return dst[1] - 2;
}
