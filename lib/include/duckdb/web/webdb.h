#ifndef INCLUDE_DUCKDB_WEB_WEBDB_H_
#define INCLUDE_DUCKDB_WEB_WEBDB_H_

#include <cstring>
#include <duckdb/main/prepared_statement.hpp>
#include <initializer_list>
#include <stdexcept>
#include <string>
#include <string_view>
#include <unordered_map>

#include "duckdb.hpp"
#include "duckdb/common/file_system.hpp"
#include "duckdb/main/query_result.hpp"
#include "duckdb/parser/parser.hpp"
#include "duckdb/web/arrow_insert_options.h"
#include "duckdb/web/config.h"
#include "duckdb/web/io/buffered_filesystem.h"
#include "duckdb/web/io/default_filesystem.h"
#include "duckdb/web/io/file_page_buffer.h"
#include "duckdb/web/io/file_stats.h"
#include "nonstd/span.h"

namespace duckdb {
namespace web {

struct BufferingArrowIPCStreamDecoder;

class WebDB {
   public:
    /// A connection
    class Connection {
       protected:
        /// The webdb
        WebDB& webdb_;
        /// The connection
        duckdb::Connection connection_;

        /// The current result (if any)
        std::unique_ptr<duckdb::QueryResult> current_query_result_ = nullptr;
        /// The current arrow schema (if any)
        std::shared_ptr<arrow::Schema> current_schema_ = nullptr;
        /// The current patched arrow schema (if any)
        std::shared_ptr<arrow::Schema> current_schema_patched_ = nullptr;
        /// The currently active prepared statements
        std::unordered_map<size_t, std::unique_ptr<duckdb::PreparedStatement>> prepared_statements_ = {};
        /// The next prepared statement id
        size_t next_prepared_statement_id_ = 0;
        /// The current arrow ipc input stream
        std::optional<ArrowInsertOptions> arrow_insert_options_;
        /// The current arrow ipc input stream
        std::unique_ptr<BufferingArrowIPCStreamDecoder> arrow_ipc_stream_;

        // Fully materialize a given result set and return it as an Arrow Buffer
        arrow::Result<std::shared_ptr<arrow::Buffer>> MaterializeQueryResult(
            std::unique_ptr<duckdb::QueryResult> result);
        // Setup streaming of a result set and return the schema as an Arrow Buffer
        arrow::Result<std::shared_ptr<arrow::Buffer>> StreamQueryResult(std::unique_ptr<duckdb::QueryResult> result);
        // Execute a prepared statement by setting up all arguments and returning the query result
        arrow::Result<std::unique_ptr<duckdb::QueryResult>> ExecutePreparedStatement(size_t statement_id,
                                                                                     std::string_view args_json);

       public:
        /// Constructor
        Connection(WebDB& webdb);
        /// Destructor
        ~Connection();

        /// Get a connection
        auto& connection() { return connection_; }
        /// Get the filesystem
        duckdb::FileSystem& filesystem();

        /// Run a query and return an arrow buffer
        arrow::Result<std::shared_ptr<arrow::Buffer>> RunQuery(std::string_view text);
        /// Send a query and return an arrow buffer
        arrow::Result<std::shared_ptr<arrow::Buffer>> SendQuery(std::string_view text);
        /// Fetch query results and return an arrow buffer
        arrow::Result<std::shared_ptr<arrow::Buffer>> FetchQueryResults();

        /// Prepare a statement and return its identifier
        arrow::Result<size_t> CreatePreparedStatement(std::string_view text);
        /// Execute a prepared statement with the given parameters in stringifed json format and return full result
        arrow::Result<std::shared_ptr<arrow::Buffer>> RunPreparedStatement(size_t statement_id,
                                                                           std::string_view args_json);
        /// Execute a prepared statement with the given parameters in stringifed json format and stream result
        arrow::Result<std::shared_ptr<arrow::Buffer>> SendPreparedStatement(size_t statement_id,
                                                                            std::string_view args_json);
        /// Close a prepared statement by its identifier
        arrow::Status ClosePreparedStatement(size_t statement_id);

        /// Insert an arrow record batch from an IPC stream
        arrow::Status InsertArrowFromIPCStream(nonstd::span<const uint8_t> stream, std::string_view options);
        /// Insert csv data from a path
        arrow::Status InsertCSVFromPath(std::string_view path, std::string_view options);
        /// Insert json data from a path
        arrow::Status InsertJSONFromPath(std::string_view path, std::string_view options);
    };

   protected:
    /// The buffer manager
    std::shared_ptr<io::FilePageBuffer> file_page_buffer_;
    /// The buffered filesystem
    io::BufferedFileSystem* buffered_filesystem_;
    /// The (shared) database
    std::shared_ptr<duckdb::DuckDB> database_;
    /// The connections
    std::unordered_map<Connection*, std::unique_ptr<Connection>> connections_;

    /// The file statistics (if any)
    std::shared_ptr<io::FileStatisticsRegistry> file_stats_ = {};
    /// The pinned web files (if any)
    std::unordered_map<std::string_view, std::unique_ptr<io::WebFileSystem::WebFileHandle>> pinned_web_files_ = {};
    /// The config
    WebDBConfig config_;

   public:
    /// Constructor
    WebDB(std::string_view path = "", std::unique_ptr<duckdb::FileSystem> fs = io::CreateDefaultFileSystem());
    /// Destructor
    ~WebDB();

    /// Get the filesystem
    auto& filesystem() { return database_->GetFileSystem(); }
    /// Get the database
    auto& database() { return *database_; }
    /// Get the buffer manager
    auto& file_page_buffer() { return *file_page_buffer_; }

    /// Get the version
    std::string_view GetVersion();
    /// Get the feature flags
    uint32_t GetFeatureFlags();

    /// Tokenize a script and return tokens as json
    std::string Tokenize(std::string_view text);

    /// Create a connection
    Connection* Connect();
    /// End a connection
    void Disconnect(Connection* connection);
    /// Reset the database
    arrow::Status Reset();
    /// Open a database
    arrow::Status Open(std::string_view path);

    /// Register a file URL
    arrow::Status RegisterFileURL(std::string_view file_name, std::string_view file_url,
                                  std::optional<uint64_t> file_size);
    /// Register a file URL
    arrow::Status RegisterFileBuffer(std::string_view file_name, std::unique_ptr<char[]> buffer, size_t buffer_length);
    /// Flush all file buffers
    void FlushFiles();
    /// Flush file by path
    void FlushFile(std::string_view path);
    /// Drop all files
    arrow::Status DropFiles();
    /// Drop a file
    arrow::Status DropFile(std::string_view file_name);
    /// Set a file descriptor
    arrow::Status SetFileDescriptor(uint32_t file_id, uint32_t fd);
    /// Set a file descriptor
    arrow::Result<std::string> GetFileInfo(uint32_t file_id);
    /// Copy a file to a buffer
    arrow::Result<std::shared_ptr<arrow::Buffer>> CopyFileToBuffer(std::string_view path);
    /// Copy a file to a path
    arrow::Status CopyFileToPath(std::string_view path, std::string_view out);

    /// Collect file statistics
    arrow::Status CollectFileStatistics(std::string_view path, bool enable);
    /// Export file statistics
    arrow::Result<std::shared_ptr<arrow::Buffer>> ExportFileStatistics(std::string_view path);

    /// Get the static webdb instance
    static WebDB& Get();
};

}  // namespace web
}  // namespace duckdb

#endif  // INCLUDE_DUCKDB_WEB_WEBDB_H_
