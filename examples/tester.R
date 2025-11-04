library(stringr)
library(dplyr)
library(tidyr)
library(kableExtra)

parse_music_data <- function(file_path) {
  # Read the file
  lines <- readLines(file_path)

  # Initialize storage
  configurations <- list()
  test_cases <- list()
  results <- list()

  # Current configuration
  current_config <- NULL
  config_counter <- 0

  for (line in lines) {
    line <- trimws(line)
    if (line == "") next

    # Parse Detector Configuration line
    if (str_detect(line, "^Detector Configuration:")) {
      config_counter <- config_counter + 1
      config_parts <- str_split(line, "\\s+")[[1]]
      current_config <- list(
        config_id = config_counter,
        size = as.numeric(config_parts[3]),
        padding = as.numeric(config_parts[4]),
        power = as.numeric(config_parts[5]),
        clarity = as.numeric(config_parts[6]),
        method = config_parts[7]
      )
      configurations[[config_counter]] <- current_config
    }else if (str_detect(line, "^Test case")) {
      ## Parse Test case line
      test_parts <- str_split(line, "\\s+")[[1]]
      test_case <- list(
        config_id = config_counter,
        test_index = as.numeric(test_parts[3]),
        note = test_parts[4],  # e.g., "E/2"
        offset = as.numeric(test_parts[5]),
        file_path = test_parts[6]
      )
      test_cases[[length(test_cases) + 1]] <- test_case
    }else if (str_detect(line, "^Result:")) {
      # Parse Result line
      result_parts <- str_split(line, "\\s+")[[1]]
      print(c(">Result: ",  line))
      # Extract note information (e.g., "E/E")
      note_info <- result_parts[3]
      note_parts <- str_split(note_info, "/")[[1]]
      actual_note <- note_parts[1]
      detected_note <- note_parts[2]
      print(c(">Result: notes: ", note_parts, actual_note, detected_note))

      # Extract octave information (e.g., "2/2")
      octave_info <- result_parts[4]
      octave_parts <- str_split(octave_info, "/")[[1]]
      actual_octave <- as.numeric(octave_parts[1])
      detected_octave <- as.numeric(octave_parts[2])

      # Extract cents information (e.g., "0.000/0.572")
      cents_info <- result_parts[5]
      cents_parts <- str_split(cents_info, "/")[[1]]
      actual_cents <- as.numeric(cents_parts[1])
      detected_cents <- as.numeric(cents_parts[2])

      result <- list(
        config_id = config_counter,
        result_index = as.numeric(result_parts[2]),
        actual_note = actual_note,
        detected_note = detected_note,
        actual_octave = actual_octave,
        detected_octave = detected_octave,
        actual_cents = actual_cents,
        detected_cents = detected_cents
      )
      results[[length(results) + 1]] <- result
    }
  }

  # Convert to data frames
  config_df <- do.call(rbind, lapply(configurations, as.data.frame))
  test_cases_df <- do.call(rbind, lapply(test_cases, as.data.frame))
  results_df <- do.call(rbind, lapply(results, as.data.frame))

  # Extract note and octave from test cases
  test_cases_df <- test_cases_df %>%
    mutate(
      actual_note = str_split(note, "/") %>% sapply(function(x) x[1]),
      actual_octave = str_split(note, "/") %>% sapply(function(x) as.numeric(x[2]))
    ) %>%
    select(-note)

  # Merge test case information with results
  analysis_df <- results_df %>%
    left_join(test_cases_df, by = c("config_id", "result_index" = "test_index")) %>%
    left_join(config_df, by = "config_id")

  (list(
    configurations = config_df,
    test_cases = test_cases_df,
    results = results_df,
    analysis_data = analysis_df
  ))
}

# Usage example:
# data_frames <- parse_music_data("your_data_file.txt")

# If you want to work with the text directly instead of a file:
parse_music_text <- function(text) {
  # Write text to temporary file and parse
  temp_file <- tempfile()
  writeLines(text, temp_file)
  result <- parse_music_data(temp_file)
  file.remove(temp_file)
  return(result)
}
data_frames <- parse_music_data("tester.log")
## # Example with your sample data
## sample_data <- "Detector Configuration:  1024  256  1.000  0.300 McLeod
## Test case   1 E/2  0.000 examples/data/E2_0.raw
## Result:   1 E/E 2/2  0.000/ 0.572
## Result:   1 E/E 2/2  0.000/ 2.272
## Result:   1 E/F 2/3  0.000/-44.262
## Test case   2 A/2  0.000 examples/data/A2_0.raw
## Result:   2 A/A 2/3  0.000/ 0.312
## Result:   2 A/A 2/2  0.000/ 0.473
## Test case   3 D/3  0.000 examples/data/D3_0.raw
## Result:   3 D/D 3/4  0.000/-32.501
## Result:   3 D/D 3/4  0.000/10.215
## Detector Configuration:  1024  256  1.000  0.500 McLeod
## Test case   1 E/2  0.000 examples/data/E2_0.raw
## Test case   2 A/2  0.000 examples/data/A2_0.raw
## Result:   2 A/A 2/2  0.000/ 2.406
## Result:   2 A/A 2/2  0.000/ 2.688
## Result:   2 A/A# 2/4  0.000/ 0.300"

## # Parse the sample data
## data_frames <- parse_music_text(sample_data)

# Access the data frames
configurations <- data_frames$configurations
test_cases <- data_frames$test_cases
results <- data_frames$results
analysis_data <- data_frames$analysis_data

## # Print the structure of each data frame
## print("Configurations:")
## print(configurations)

## print("Test Cases:")
## print(test_cases)

## print("Results:")
## print(results)

## print("Analysis Data (combined):")
## print(analysis_data)

# You can now analyze the data, for example:
# - Calculate detection accuracy
# - Analyze cents deviation by note and configuration
# - Compare different detector configurations
generate_report <- function(data_frames) {
  analysis_data <- data_frames$analysis_data

  # Report for each configuration
  config_report <- analysis_data %>%
    group_by(config_id, size, padding, power, clarity, method) %>%
    summarise(
      # Basic counts
      total_results = n(),

      # Note detection accuracy
      incorrect_note_detections = sum(actual_note.x != detected_note),
      correct_note_detections = sum(actual_note.x == detected_note),
      accuracy_rate = correct_note_detections / total_results,

      # Statistical properties of detected cents
      cents_min = min(detected_cents, na.rm = TRUE),
      cents_max = max(detected_cents, na.rm = TRUE),
      cents_mean = mean(detected_cents, na.rm = TRUE),
      cents_median = median(detected_cents, na.rm = TRUE),
      cents_variance = var(detected_cents, na.rm = TRUE),
      cents_sd = sd(detected_cents, na.rm = TRUE),
      cents_q1 = quantile(detected_cents, 0.25, na.rm = TRUE),
      cents_q3 = quantile(detected_cents, 0.75, na.rm = TRUE),

      .groups = 'drop'
    )

  # Detailed breakdown by note for each configuration
  note_breakdown <- analysis_data %>%
    group_by(config_id, actual_note.x) %>%
    summarise(
      total_tests = n(),
      incorrect_detections = sum(actual_note.x != detected_note),
      accuracy_rate = sum(actual_note.x == detected_note) / n(),

      # Cents statistics for this note
      cents_mean = mean(detected_cents, na.rm = TRUE),
      cents_sd = sd(detected_cents, na.rm = TRUE),
      cents_min = min(detected_cents, na.rm = TRUE),
      cents_max = max(detected_cents, na.rm = TRUE),

      .groups = 'drop'
    )

  # Incorrect detections breakdown (what was detected instead)
  incorrect_detections <- analysis_data %>%
    filter(actual_note.x != detected_note) %>%
    group_by(config_id, actual_note.x, detected_note) %>%
    summarise(
      count = n(),
      avg_cents_error = mean(detected_cents, na.rm = TRUE),
      .groups = "drop"
    )

  (list(
    config_summary = config_report,
    note_breakdown = note_breakdown,
    incorrect_detections = incorrect_detections
  ))
}

# Generate the report
report <- generate_report(data_frames)

# Print the report in a nice format
cat("=== MUSIC DETECTION ANALYSIS REPORT ===\n\n")

# Configuration Summary
cat("## CONFIGURATION SUMMARY\n")
print(report$config_summary %>%
        select(config_id, size, padding, power, clarity, method, total_results,
               correct_note_detections, incorrect_note_detections,
               accuracy_rate,
               cents_mean, cents_median, cents_sd, cents_min, cents_max))
cat("\n\n")

# Detailed note breakdown for each configuration
cat("## NOTE-BY-NOTE BREAKDOWN\n")
for (config_id in unique(report$note_breakdown$config_id)) {
  cat(sprintf("\n### Configuration %d\n", config_id))
  config_notes <- report$note_breakdown %>%
    filter(config_id == !!config_id) %>%
    select(actual_note.x, total_tests, incorrect_detections, accuracy_rate,
           cents_mean, cents_sd)
  print(config_notes)
  cat("\n")
}

# Incorrect detections analysis
cat("## INCORRECT DETECTION ANALYSIS\n")
for (config_id in unique(report$incorrect_detections$config_id)) {
  cat(sprintf("\n### Configuration %d - Incorrect Detections\n", config_id))
  incorrect_config <- report$incorrect_detections %>%
    filter(config_id == !!config_id) %>%
    arrange(actual_note.x, desc(count))

  if (nrow(incorrect_config) > 0) {
    print(incorrect_config)
  } else {
    cat("No incorrect detections for this configuration.\n")
  }
  cat("\n")
}

# Additional detailed statistical summary
cat("## DETAILED STATISTICAL SUMMARY\n")
for (config_id in unique(report$config_summary$config_id)) {
  config_data <- analysis_data %>% filter(config_id == !!config_id)
  config_info <- report$config_summary %>% filter(config_id == !!config_id)

  cat(sprintf("\n### Configuration %d (%s)\n",
              config_id,
              paste(config_info$size, config_info$padding, config_info$power,
                    config_info$clarity, config_info$method)))

  cat(sprintf("Total results: %d\n", config_info$total_results))
  cat(sprintf("Note detection accuracy: %.2f%%\n",
              config_info$accuracy_rate * 100))
  cat(sprintf("Cents statistics (all detections):\n"))
  cat(sprintf("  Mean: %.3f, Median: %.3f, SD: %.3f\n",
              config_info$cents_mean, config_info$cents_median,
              config_info$cents_sd))
  cat(sprintf("  Range: [%.3f, %.3f], IQR: [%.3f, %.3f]\n",
              config_info$cents_min, config_info$cents_max,
              config_info$cents_q1, config_info$cents_q3))
  cat(sprintf("  Variance: %.3f\n", config_info$cents_variance))

  # Cents statistics for correct detections only
  correct_cents <- config_data %>%
    filter(actual_note.x == detected_note) %>%
    pull(detected_cents)

  if (length(correct_cents) > 0) {
    cat(sprintf("Cents statistics (correct detections only):\n"))
    cat(sprintf("  Mean: %.3f, Median: %.3f, SD: %.3f\n",
                mean(correct_cents), median(correct_cents), sd(correct_cents)))
    cat(sprintf("  Range: [%.3f, %.3f]\n", min(correct_cents),
                max(correct_cents)))
  }
  cat("\n")
}

# Optional: Create visualizations
## if (nrow(analysis_data) > 0) {
##   library(ggplot2)

##   # Plot 1: Accuracy by configuration
##   p1 <- ggplot(report$config_summary, aes(x = factor(config_id),
##                                           y = accuracy_rate)) +
##     geom_col(fill = "steelblue") +
##     labs(title = "Note Detection Accuracy by Configuration",
##          x = "Configuration ID", y = "Accuracy Rate") +
##     theme_minimal()
##   print(p1)

##   # Plot 2: Cents distribution by configuration
##   p2 <- ggplot(analysis_data, aes(x = factor(config_id), y = detected_cents)) +
##     geom_boxplot(fill = "lightgreen") +
##     labs(title = "Detected Cents Distribution by Configuration",
##          x = "Configuration ID", y = "Cents") +
##     theme_minimal()
##   print(p2)

##   # Plot 3: Cents distribution by actual note (for correct detections)
##   correct_detections <- analysis_data %>% filter(actual_note.x == detected_note)
##   if (nrow(correct_detections) > 0) {
##     p3 <- ggplot(correct_detections, aes(x = actual_note.x, y = detected_cents)) +
##       geom_boxplot(fill = "lightcoral") +
##       facet_wrap(~config_id) +
##       labs(title = "Cents Distribution by Note (Correct Detections Only)",
##            x = "Actual Note", y = "Cents") +
##       theme_minimal()
##     print(p3)
##   }
## }
