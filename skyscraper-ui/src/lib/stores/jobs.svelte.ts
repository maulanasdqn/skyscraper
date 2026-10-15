import type { Job, PaginatedResponse, JobStatus } from '../types'
import { listJobs as fetchJobsApi, getJob as fetchJobApi } from '../api/skyscraper'

let jobs = $state<Job[]>([])
let selectedJob = $state<Job | null>(null)
let loading = $state(false)
let error = $state<string | null>(null)
let page = $state(1)
let perPage = $state(20)
let total = $state(0)
let totalPages = $state(0)
let statusFilter = $state<JobStatus | ''>('')

export function getJobs() {
  return jobs
}

export function getSelectedJob() {
  return selectedJob
}

export function isLoading() {
  return loading
}

export function getError() {
  return error
}

export function getPage() {
  return page
}

export function getTotalPages() {
  return totalPages
}

export function getTotal() {
  return total
}

export function getStatusFilter() {
  return statusFilter
}

export function setStatusFilter(s: JobStatus | '') {
  statusFilter = s
  page = 1
}

export function setPage(p: number) {
  page = p
}

export async function fetchJobs(apiKey: string) {
  loading = true
  error = null
  try {
    const params: Record<string, unknown> = { page, per_page: perPage }
    if (statusFilter) params.status = statusFilter
    const res: PaginatedResponse<Job> = await fetchJobsApi(apiKey, params as never)
    jobs = res.data
    total = res.pagination.total
    totalPages = res.pagination.total_pages
  } catch (e) {
    error = e instanceof Error ? e.message : 'Failed to fetch jobs'
  } finally {
    loading = false
  }
}

export async function fetchJob(apiKey: string, id: string) {
  loading = true
  error = null
  try {
    selectedJob = await fetchJobApi(apiKey, id)
  } catch (e) {
    error = e instanceof Error ? e.message : 'Failed to fetch job'
    selectedJob = null
  } finally {
    loading = false
  }
}

export function clearSelectedJob() {
  selectedJob = null
}

export function clearError() {
  error = null
}
