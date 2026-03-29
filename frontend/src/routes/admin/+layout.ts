import type { LayoutLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { getToken, isTokenExpired } from '$lib/admin-api';

export const ssr = false;

export const load: LayoutLoad = ({ url }) => {
  if (url.pathname === '/admin/login') return {};

  const token = getToken();
  if (!token || isTokenExpired()) {
    throw redirect(302, '/admin/login');
  }

  return {};
};
