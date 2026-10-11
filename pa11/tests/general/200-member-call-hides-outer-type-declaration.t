namespace n {

struct fields {
};

struct buffer {
  int prepared;

  buffer() : prepared(0) {
  }

  void prepare() {
    prepared = 1;
  }
};

struct fuzz {
  void fields(buffer & db) {
    db.prepared = 7;
  }

  void response(buffer & db) {
    fields(db);
    db.prepare();
  }
};

}

int main() {
  n::fuzz value;
  n::buffer db;
  value.response(db);
  return db.prepared == 1 ? 0 : 1;
}
