#define MAKE_WRAPPED_TYPE(name) \
  typedef library::type_of::remove_cv_ref_t<decltype( \
      library::detail::wrap(library::detail::deref( \
          missing_result, static_cast<tag>(0))))> name
