inline void vector_mask_literal() {
  (void)((__attribute__((__vector_size__(16))) int){1, 2, 3, 4});
}

int anchor;
