import { FileNode } from '../types';

export const MAIN_FLOW_YAML = `name: User Management Flow
description: Demonstrates GET, POST, and data extraction with JSONPlaceholder API
config:
  baseUrl: https://jsonplaceholder.typicode.com
  timeout: 10000
  # Note: Headers removed from global config to avoid issues with GET requests
  # We apply Content-Type only on POST/PUT steps

beforeTest:
  - name: Check Service Status
    method: GET
    url: /posts/1
    verify:
      status: 200

steps:
  # 1. Get User Information
  - name: Get User Details
    method: GET
    url: /users/1
    extract:
      user_id: body.id
      user_name: body.name
      user_email: body.email
    verify:
      status: 200
      body.id: 1

  # 2. Create New Post
  - name: Create New Post
    method: POST
    url: /posts
    headers:
      Content-Type: application/json
    body:
      title: {{$mock.title}}
      body: {{$mock.text}}
      userId: {{user_id}}
    extract:
      post_id: body.id
      post_title: body.title
    verify:
      status: 201
      body.userId: {{user_id}}
      # Note: JSONPlaceholder returns fake ID (101) but doesn't persist data

  # 3. Verify API Still Works
  # Note: JSONPlaceholder doesn't persist POST data, so GET /posts/{{post_id}} would return 404
  # Instead, we verify the API is still working by getting all posts
  - name: Get All Posts (Verify API Works)
    method: GET
    url: /posts
    verify:
      status: 200

afterTest:
  - name: Cleanup - Get All Posts
    method: GET
    url: /posts
    verify:
      status: 200
`;

export const AUTH_FLOW_YAML = `name: Data Retrieval Module
description: Example flow demonstrating GET requests and data extraction
steps:
  - name: Get Post by ID
    method: GET
    url: https://jsonplaceholder.typicode.com/posts/1
    extract:
      # Extract data for use in other steps
      post_title: body.title
      post_body: body.body
      author_id: body.userId
    verify:
      status: 200
      body.id: 1

  - name: Get User by ID
    method: GET
    url: https://jsonplaceholder.typicode.com/users/{{author_id}}
    verify:
      status: 200
      body.id: {{author_id}}
`;

export const getInitialFiles = (): FileNode[] => {
  return [
    {
      id: 'root',
      name: 'Nexus Project',
      type: 'folder',
      isOpen: true,
      children: [
        { id: '1', name: 'master_flow.yaml', type: 'file', content: MAIN_FLOW_YAML },
        { id: '2', name: 'auth_flow.yaml', type: 'file', content: AUTH_FLOW_YAML },
      ],
    },
  ];
};

