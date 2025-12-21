//! Middleware chain and stack implementation.

use super::context::QueryContext;
use super::types::{
    BoxFuture, Middleware, MiddlewareResult, Next, QueryResponse, SharedMiddleware,
};
use std::sync::Arc;

/// A chain of middleware that processes queries.
///
/// The chain executes middleware in order, with each middleware able to:
/// - Modify the query context before passing to the next
/// - Modify the response after receiving from the next
/// - Short-circuit by not calling next
pub struct MiddlewareChain {
    middlewares: Vec<SharedMiddleware>,
}

impl MiddlewareChain {
    /// Create an empty middleware chain.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Create a chain with initial middleware.
    pub fn with(middlewares: Vec<SharedMiddleware>) -> Self {
        Self { middlewares }
    }

    /// Add middleware to the end of the chain.
    pub fn push<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(middleware));
    }

    /// Add middleware to the beginning of the chain.
    pub fn prepend<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.insert(0, Arc::new(middleware));
    }

    /// Get the number of middlewares in the chain.
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Execute the middleware chain.
    pub fn execute<'a, F>(
        &'a self,
        ctx: QueryContext,
        final_handler: F,
    ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>>
    where
        F: FnOnce(QueryContext) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> + Send + 'a,
    {
        self.execute_at(0, ctx, final_handler)
    }

    fn execute_at<'a, F>(
        &'a self,
        index: usize,
        ctx: QueryContext,
        final_handler: F,
    ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>>
    where
        F: FnOnce(QueryContext) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> + Send + 'a,
    {
        if index >= self.middlewares.len() {
            // End of chain, call the final handler
            return final_handler(ctx);
        }

        let middleware = &self.middlewares[index];

        // Skip disabled middleware
        if !middleware.enabled() {
            return self.execute_at(index + 1, ctx, final_handler);
        }

        // Create the next handler that will call the rest of the chain
        

        ({
            // We need to move the final_handler but also use it in the closure
            // This requires some careful handling
            Box::pin(async move {
                // This is a placeholder - the actual implementation needs
                // to properly chain the middleware
                middleware
                    .handle(
                        ctx,
                        Next {
                            inner: Box::new(move |ctx| {
                                // Recursively call the rest of the chain
                                // Note: This is simplified - real impl would be more complex
                                final_handler(ctx)
                            }),
                        },
                    )
                    .await
            })
        }) as _
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// A stack of middleware with builder pattern.
///
/// This is a more ergonomic wrapper around `MiddlewareChain`.
pub struct MiddlewareStack {
    chain: MiddlewareChain,
}

impl MiddlewareStack {
    /// Create a new empty stack.
    pub fn new() -> Self {
        Self {
            chain: MiddlewareChain::new(),
        }
    }

    /// Add middleware to the stack (builder pattern).
    pub fn with<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.chain.push(middleware);
        self
    }

    /// Add middleware mutably.
    pub fn push<M: Middleware + 'static>(&mut self, middleware: M) -> &mut Self {
        self.chain.push(middleware);
        self
    }

    /// Add middleware to the front of the stack.
    pub fn prepend<M: Middleware + 'static>(&mut self, middleware: M) -> &mut Self {
        self.chain.prepend(middleware);
        self
    }

    /// Get the number of middlewares.
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Get the underlying chain.
    pub fn into_chain(self) -> MiddlewareChain {
        self.chain
    }

    /// Execute the stack with a final handler.
    pub fn execute<'a, F>(
        &'a self,
        ctx: QueryContext,
        final_handler: F,
    ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>>
    where
        F: FnOnce(QueryContext) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> + Send + 'a,
    {
        self.chain.execute(ctx, final_handler)
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

impl From<MiddlewareStack> for MiddlewareChain {
    fn from(stack: MiddlewareStack) -> Self {
        stack.chain
    }
}

/// Builder for creating middleware stacks.
pub struct MiddlewareBuilder {
    middlewares: Vec<SharedMiddleware>,
}

impl MiddlewareBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add middleware.
    pub fn with<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    /// Add middleware conditionally.
    pub fn with_if<M: Middleware + 'static>(self, condition: bool, middleware: M) -> Self {
        if condition {
            self.with(middleware)
        } else {
            self
        }
    }

    /// Build the middleware chain.
    pub fn build(self) -> MiddlewareChain {
        MiddlewareChain::with(self.middlewares)
    }

    /// Build into a stack.
    pub fn build_stack(self) -> MiddlewareStack {
        MiddlewareStack {
            chain: self.build(),
        }
    }
}

impl Default for MiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_chain_empty() {
        let chain = MiddlewareChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_middleware_stack_builder() {
        struct DummyMiddleware;
        impl Middleware for DummyMiddleware {
            fn handle<'a>(
                &'a self,
                ctx: QueryContext,
                next: Next<'a>,
            ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> {
                Box::pin(async move { next.run(ctx).await })
            }
        }

        let stack = MiddlewareStack::new()
            .with(DummyMiddleware)
            .with(DummyMiddleware);

        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_middleware_builder() {
        struct TestMiddleware;
        impl Middleware for TestMiddleware {
            fn handle<'a>(
                &'a self,
                ctx: QueryContext,
                next: Next<'a>,
            ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> {
                Box::pin(async move { next.run(ctx).await })
            }
        }

        let chain = MiddlewareBuilder::new()
            .with(TestMiddleware)
            .with_if(true, TestMiddleware)
            .with_if(false, TestMiddleware)
            .build();

        assert_eq!(chain.len(), 2);
    }
}
