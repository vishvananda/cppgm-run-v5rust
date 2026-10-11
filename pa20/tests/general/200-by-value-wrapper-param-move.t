struct MoveMember {
  int value;
  MoveMember();
  MoveMember(const MoveMember &);
  MoveMember(MoveMember &&);
  ~MoveMember();
};

struct MoveBox {
  MoveMember member;
  MoveBox();
  MoveBox(const MoveBox &);
  MoveBox(MoveBox &&);
  ~MoveBox();
};

struct CopyMember {
  int value;
  CopyMember();
  CopyMember(const CopyMember &);
  ~CopyMember();
};

struct CopyBox {
  CopyMember member;
  CopyBox();
  CopyBox(const CopyBox & other) : member(other.member) {}
  ~CopyBox();
};

int take_move_box(MoveBox box) { return 1; }

int take_copy_box(CopyBox box) { return 2; }
