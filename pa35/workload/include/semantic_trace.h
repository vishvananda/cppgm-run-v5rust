#pragma once

#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "semantic_errors.h"

namespace semantic_model {
struct SourceDeclAnchorCache;
struct Scope;
struct ClassInfo;
struct FunctionBinding;
struct FunctionTemplateDecl;
struct ClassTemplateDecl;
struct ValueBinding;
struct AliasTemplateDecl;
struct VariableTemplateDecl;
}

class SemanticContext;
struct CppAstNode;

class DiagnosticContext
{
public:
  struct Frame
  {
    mutable bool realized = false;
    mutable std::string entry;
    const void * lazy_guard = nullptr;
    std::string (*lazy_realize)(const void *) = nullptr;
  };

  static thread_local std::vector<Frame> stack_;
  static thread_local std::vector<std::string> captured_stack_;

  static void update_top_lazy_guard(const void * old_guard,
                                    const void * new_guard);
  static void log_push_current();
  static void log_pop_current();

  struct Guard
  {
    explicit Guard(const std::string & entry) : unwinding_on_entry_(std::uncaught_exception())
    {
      if(!unwinding_on_entry_) {
        captured_stack_.clear();
      }
      Frame frame;
      frame.realized = true;
      frame.entry = entry;
      stack_.push_back(frame);
      log_push();
    }

    ~Guard();

    Guard(const Guard &) = delete;
    Guard & operator=(const Guard &) = delete;

  private:
    bool unwinding_on_entry_;

    void log_push();
  };

  template<typename Fn>
  struct LazyGuard
  {
    explicit LazyGuard(Fn fn)
      : fn_(std::move(fn)),
        unwinding_on_entry_(std::uncaught_exception()),
        active_(true)
    {
      if(!unwinding_on_entry_) {
        captured_stack_.clear();
      }
      Frame frame;
      frame.lazy_guard = this;
      frame.lazy_realize = &LazyGuard::realize_frame;
      stack_.push_back(frame);
      log_push();
    }

    LazyGuard(LazyGuard && other)
      : fn_(std::move(other.fn_)),
        unwinding_on_entry_(other.unwinding_on_entry_),
        active_(other.active_)
    {
      if(active_) {
        update_top_lazy_guard(&other, this);
        other.active_ = false;
      }
    }

    ~LazyGuard()
    {
      if(!active_) {
        return;
      }
      if(!stack_.empty() &&
         std::uncaught_exception() &&
         !unwinding_on_entry_ &&
         stack_.size() >= captured_stack_.size()) {
        captured_stack_ = realize_stack(stack_);
      }
      log_pop_current();
      if(!stack_.empty()) {
        stack_.pop_back();
      }
    }

    LazyGuard(const LazyGuard &) = delete;
    LazyGuard & operator=(const LazyGuard &) = delete;

  private:
    Fn fn_;
    bool unwinding_on_entry_;
    bool active_;

    static std::string realize_frame(const void * guard)
    {
      return static_cast<const LazyGuard *>(guard)->fn_();
    }

    void log_push()
    {
      log_push_current();
    }
  };

  template<typename Fn>
  static LazyGuard<Fn> make_guard(Fn fn)
  {
    return LazyGuard<Fn>(std::move(fn));
  }

  static void clear();
  static std::string current_frame();
  static std::string format_stack();
  static std::string format_stack_compact(std::size_t max_frames,
                                          std::size_t max_line_chars);
  static std::string realize_frame(const Frame & frame);
  static std::vector<std::string> realize_stack(const std::vector<Frame> & frames);
};

#define DIAG_CONTEXT(msg) \
  auto diag_guard_##__LINE__ = \
      DiagnosticContext::make_guard([&]() -> std::string { return (msg); })

namespace semantic_trace {

std::string scope_name_for_diagnostic(const semantic_model::Scope & scope);
std::string scope_bindings_for_diagnostic(const semantic_model::Scope & scope);
std::string node_location_note(const SemanticContext & ctx,
                               const char * label,
                               const CppAstNode * node);
std::string current_location_note(const SemanticContext & ctx,
                                  const CppAstNode * node);
std::string previous_function_location_note(const SemanticContext & ctx,
                                            const char * label,
                                            const semantic_model::FunctionBinding * binding);
bool source_location_points_at_identifier(const std::string & location,
                                          const std::string & identifier);
std::string name_location_in_node(const SemanticContext & ctx,
                                  const CppAstNode * node,
                                  const std::string & unqualified_name,
                                  bool prefer_last_name);
const semantic_model::SourceDeclAnchorCache & function_template_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::FunctionTemplateDecl * decl);
const semantic_model::SourceDeclAnchorCache & function_binding_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding);
const semantic_model::SourceDeclAnchorCache & class_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::ClassInfo * info);
const semantic_model::SourceDeclAnchorCache & class_template_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::ClassTemplateDecl * decl);
const semantic_model::SourceDeclAnchorCache & value_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::ValueBinding * binding);
const semantic_model::SourceDeclAnchorCache & alias_template_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::AliasTemplateDecl * decl);
const semantic_model::SourceDeclAnchorCache & variable_template_decl_anchor(
    const SemanticContext & ctx,
    const semantic_model::VariableTemplateDecl * decl);
std::string function_template_name_location(
    const SemanticContext & ctx,
    const semantic_model::FunctionTemplateDecl * decl);
std::string template_decl_primary_location(const SemanticContext & ctx,
                                           const semantic_model::FunctionTemplateDecl * decl);
std::string template_decl_location_details(const SemanticContext & ctx,
                                           const semantic_model::FunctionTemplateDecl * decl);
std::string function_template_signature_for_diagnostic(
    const semantic_model::FunctionTemplateDecl & decl);
std::string previous_value_location_note(const SemanticContext & ctx,
                                         const char * label,
                                         const semantic_model::ValueBinding * binding);
std::string previous_class_location_note(const SemanticContext & ctx,
                                         const char * label,
                                         const semantic_model::ClassInfo * info);
std::string compact_diagnostic_message(const std::string & text,
                                       std::size_t max_line_chars);
std::string write_diagnostic_sidecar(const std::string & full_message,
                                     const std::string & full_context);
bool diagnostic_mode_compact();
bool diagnostic_mode_sidecar();
std::size_t diagnostic_max_stack_frames();
std::size_t diagnostic_max_line_chars();

template<class Fn, class ContextFn>
auto append_template_context(Fn fn, ContextFn context_fn) -> decltype(fn())
{
  try
  {
    return fn();
  }
  catch(const TemplateSubstitutionFailure & e)
  {
    throw TemplateSubstitutionFailure(
        append_diagnostic_detail(e.diagnostic(), context_fn()));
  }
  catch(const NoViableConstructorError & e)
  {
    throw NoViableConstructorError(std::string(e.what()) + context_fn());
  }
  catch(const NotDataMemberExpressionError & e)
  {
    throw NotDataMemberExpressionError(std::string(e.what()) + context_fn());
  }
  catch(const ExplicitSpecializationAfterInstantiationError & e)
  {
    throw ExplicitSpecializationAfterInstantiationError(
        std::string(e.what()) + context_fn());
  }
  catch(const std::logic_error & e)
  {
    throw std::logic_error(std::string(e.what()) + context_fn());
  }
}

template<class Fn, class ContextFn>
auto append_template_context_as_substitution_failure(Fn fn,
                                                     ContextFn context_fn) -> decltype(fn())
{
  try
  {
    return fn();
  }
  catch(const TemplateSubstitutionFailure & e)
  {
    throw TemplateSubstitutionFailure(
        append_diagnostic_detail(e.diagnostic(), context_fn()));
  }
  catch(const ExplicitSpecializationAfterInstantiationError & e)
  {
    throw ExplicitSpecializationAfterInstantiationError(
        std::string(e.what()) + context_fn());
  }
  catch(const std::logic_error & e)
  {
    throw TemplateSubstitutionFailure(make_diagnostic(DiagnosticKind::SubstitutionFailure,
                                                     std::string(e.what()) + context_fn()));
  }
}

}  // namespace semantic_trace
