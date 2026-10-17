// N3485 11 [class.access]: a qualified friend name must be accessible.
// expect reject
class Other {
  class Private;
};
class Owner {
  friend class Other::Private;
};
