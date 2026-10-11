#pragma once

struct Cursor {
  explicit Cursor(int value);
  ~Cursor();

  int size;
};

struct Parser {
  explicit Parser(int value);

  Cursor cursor;
};
